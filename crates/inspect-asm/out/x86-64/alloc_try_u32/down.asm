inspect_asm::alloc_try_u32::down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	push rax
	mov r14, rsi
	mov rbx, rdi
	mov r15, qword ptr [rsi]
	mov r12, qword ptr [r15]
	mov r13, r12
	and r13, -4
	add r13, -8
	cmp r13, qword ptr [r15 + 8]
	jb .LBB0_5
	mov qword ptr [r15], r13
.LBB0_0:
	call rdx
	mov dword ptr [r13], eax
	mov dword ptr [r13 + 4], edx
	mov rcx, qword ptr [r14]
	mov rsi, qword ptr [rcx]
	test eax, eax
	je .LBB0_2
	cmp r13, rsi
	jne .LBB0_1
	mov rdi, r15
	mov rsi, r12
	mov ebp, edx
	call qword ptr [rip + bump_scope::bump_scope_guard::Checkpoint::reset_within_chunk@GOTPCREL]
	mov edx, ebp
	mov qword ptr [r14], r15
.LBB0_1:
	mov dword ptr [rbx + 4], edx
	mov eax, 1
	jmp .LBB0_4
.LBB0_2:
	lea rax, [r13 + 4]
	cmp r13, rsi
	jne .LBB0_3
	mov qword ptr [rcx], rax
.LBB0_3:
	mov qword ptr [rbx + 8], rax
	xor eax, eax
.LBB0_4:
	mov dword ptr [rbx], eax
	mov rax, rbx
	add rsp, 8
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_5:
	mov rdi, r14
	mov r13, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rdx, r13
	mov r13, rax
	jmp .LBB0_0
