inspect_asm::alloc_try_big_ok::try_down_mut:
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
	mov r15, qword ptr [rsi]
	mov r12, qword ptr [r15]
	xor r13d, r13d
	mov rax, r12
	sub rax, 1024
	cmovae r13, rax
	mov rbx, rdi
	and r13, -512
	cmp r13, qword ptr [r15 + 8]
	jb .LBB0_4
	test r13, r13
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
	cmp dword ptr [r13], 0
	lea rcx, [r13 + 512]
	mov r12, rcx
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
	mov rax, qword ptr [r14]
	mov qword ptr [rax], rcx
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
