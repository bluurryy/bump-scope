inspect_asm::alloc_try_big_ok::try_up_mut:
	push rbp
	mov rbp, rsp
	push r15
	push r14
	push r13
	push r12
	push rbx
	and rsp, -512
	sub rsp, 2048
	mov r14, rsi
	mov rbx, rdi
	mov r15, qword ptr [rsi]
	mov r12, qword ptr [r15]
	lea r13, [r12 - 1]
	and r13, -512
	mov rax, r13
	add rax, 1536
	mov rcx, -1
	cmovae rcx, rax
	cmp rcx, qword ptr [r15 + 8]
	ja .LBB0_4
	add r13, 512
	je .LBB0_4
.LBB0_0:
	mov qword ptr [rsp + 504], r12
	lea r12, [rsp + 512]
	mov rdi, r12
	call rdx
	mov edx, 1024
	mov rdi, r13
	mov rsi, r12
	call qword ptr [rip + memcpy@GOTPCREL]
	lea rax, [r13 + 4]
	lea r12, [r13 + 512]
	cmp dword ptr [r13], 0
	cmovne r12, rax
	je .LBB0_1
	mov r13d, dword ptr [rax]
	mov rdi, r15
	mov rsi, qword ptr [rsp + 504]
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov qword ptr [r14], r15
	mov eax, 1
	jmp .LBB0_2
.LBB0_1:
	add r13, 1024
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r13
	xor eax, eax
.LBB0_2:
	mov dword ptr [rbx], eax
	mov dword ptr [rbx + 4], r13d
	mov qword ptr [rbx + 8], r12
.LBB0_3:
	mov rax, rbx
	lea rsp, [rbp - 40]
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_4:
	mov rdi, r14
	mov r13, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_reserve_sized_in_another_chunk
	mov rdx, r13
	mov r13, rax
	test rax, rax
	jne .LBB0_0
	mov dword ptr [rbx], 2
	jmp .LBB0_3
