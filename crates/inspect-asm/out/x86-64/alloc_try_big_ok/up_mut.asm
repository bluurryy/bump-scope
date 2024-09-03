inspect_asm::alloc_try_big_ok::up_mut:
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
	mov rax, qword ptr [r15]
	mov qword ptr [rsp + 504], rax
	lea r13, [rax - 1]
	and r13, -512
	mov rax, r13
	add rax, 1536
	mov rcx, -1
	cmovae rcx, rax
	cmp rcx, qword ptr [r15 + 8]
	ja .LBB0_3
	add r13, 512
.LBB0_0:
	lea r12, [rsp + 512]
	mov rdi, r12
	call rdx
	mov edx, 1024
	mov rdi, r13
	mov rsi, r12
	call qword ptr [rip + memcpy@GOTPCREL]
	test byte ptr [r13], 1
	je .LBB0_1
	mov r12d, dword ptr [r13 + 4]
	mov rdi, r15
	mov rsi, qword ptr [rsp + 504]
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov qword ptr [r14], r15
	mov dword ptr [rbx + 4], r12d
	mov eax, 1
	jmp .LBB0_2
.LBB0_1:
	lea rax, [r13 + 512]
	add r13, 1024
	mov rcx, qword ptr [r14]
	mov qword ptr [rcx], r13
	mov qword ptr [rbx + 8], rax
	xor eax, eax
.LBB0_2:
	mov dword ptr [rbx], eax
	mov rax, rbx
	lea rsp, [rbp - 40]
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_3:
	mov rdi, r14
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_reserve_sized_in_another_chunk
	mov rdx, r12
	mov r13, rax
	jmp .LBB0_0
