inspect_asm::alloc_iter_u32::try_mut_down_a:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB0_6
	mov r14, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB0_1
.LBB0_0:
	xor ebx, ebx
	jmp .LBB0_9
.LBB0_1:
	mov r15, rsi
	shl r14, 2
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, r14
	jb .LBB0_10
	add rax, 3
	and rax, -4
	je .LBB0_10
.LBB0_2:
	sub rdx, rax
	shr rdx, 2
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], rdi
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor edx, edx
	jmp .LBB0_4
.LBB0_3:
	mov dword ptr [rax + 4*rcx], ebp
	lea rdx, [rcx + 1]
	mov qword ptr [rsp + 16], rdx
	add r13, 4
	cmp r14, r13
	je .LBB0_5
.LBB0_4:
	mov ebp, dword ptr [r15 + r13]
	mov rcx, rdx
	cmp qword ptr [rsp + 24], rdx
	jne .LBB0_3
	mov rdi, r12
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_0
	mov rax, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 16]
	jmp .LBB0_3
.LBB0_5:
	mov rax, qword ptr [rsp + 24]
	test rax, rax
	je .LBB0_6
	mov rsi, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 32]
	lea rdi, [rsi + 4*rdx]
	lea rax, [rsi + 4*rax]
	not rcx
	lea rbx, [rax + 4*rcx]
	cmp rbx, rdi
	jae .LBB0_7
	mov rbx, rsi
	jmp .LBB0_8
.LBB0_6:
	xor edx, edx
	jmp .LBB0_9
.LBB0_7:
	lea rax, [4*rdx]
	mov rdi, rbx
	mov r15, rdx
	mov rdx, rax
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r15
.LBB0_8:
	mov rax, qword ptr [r14]
	mov qword ptr [rax], rbx
.LBB0_9:
	mov rax, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_10:
	mov esi, 4
	mov r12, rdi
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB0_0
	mov rdi, r12
	jmp .LBB0_2
